import re
import os
import yaml
import unittest
import subprocess

UUT_PATH = os.environ['UUT']
PROJECT_ROOT = os.environ['PROJECT_ROOT']
VERBOSE = os.environ.get('VERBOSE', "0")


class CinitTest(unittest.TestCase):

    def tearDown(self):
        child_dumps = PROJECT_ROOT + "/system-tests/child-dump/"
        for file in os.listdir(child_dumps):
            os.unlink(child_dumps + file)

    def run_cinit(self, test_dir, dump_log=False):
        cinit = subprocess.Popen([
                        UUT_PATH,
                        "--verbose",
                        "--verbose",
                        "--config",
                        test_dir + "/config"],
                stdout=subprocess.PIPE,
                stdin=subprocess.DEVNULL,
                stderr=subprocess.STDOUT,
                cwd=PROJECT_ROOT)

        cinit.wait()
        output = cinit.stdout.read().decode("utf-8").split("\n")
        cinit.stdout.close()

        self.trace = Trace(self, output)

        if VERBOSE == "1" or dump_log:
            self.trace.dump()

    def assert_on_trace(self):
        return self.trace

    @staticmethod
    def get_test_dir(path):
        return os.path.dirname(os.path.abspath(path))


class Trace:

    def __init__(self, test, trace):
        self.trace = trace
        self.test = test
        self.index = 0

    def that(self, assertion):
        while self.index < len(self.trace):
            if assertion.matches(self.trace[self.index]):
                self.index = self.index + 1
                return self

            self.index = self.index + 1

        for line in self.trace[self.index:]:
            print(line)

        self.dump()
        self.test.fail("Event '" + str(assertion) + "' has not occured")

    def then(self, assertion):
        return self.that(assertion)

    def restart_trace(self):
        self.index = 0
        return self

    def dump(self):
        print("")
        for line in self.trace:
            print(line)


class Assert:

    def matches(self, logline):
        return False


class Sequential(Assert):

    def __init__(self, *args):
        self.matchers = list(args)

    def __str__(self):
        result = "Sequential(\n"
        for matcher in self.matchers:
            result += "    " + str(matcher) + "\n"

        return result

    def matches(self, logline):
        if self.matchers[0].matches(logline):
            self.matchers.pop(0)

        return len(self.matchers) == 0


class AnyOf(Assert):

    def __init__(self, *args):
        self.matchers = list(args)

    def __str__(self):
        result = "Alternative(\n"
        for matcher in self.matchers:
            result += "    " + str(matcher) + "\n"

        return result

    def matches(self, logline):
        for matcher in self.matchers:
            if matcher.matches(logline):
                return True
        return False


class Parallel(Assert):

    def __init__(self, *args):
        self.matchers = list(args)

    def __str__(self):
        result = "Parallel(\n"
        for matcher in self.matchers:
            result += "    " + str(matcher) + "\n"

        return result

    def matches(self, logline):
        success_indices = []
        for i in range(0, len(self.matchers)):
            if self.matchers[i].matches(logline):
                success_indices.append(i)

        for i in reversed(success_indices):
            self.matchers.pop(i)

        return len(self.matchers) == 0


class RegexMatcher(Assert):

    def __init__(self, regex):
        self.regex = regex

    def __str__(self):
        return self.regex

    def matches(self, logline):
        if re.fullmatch(
                ".*TRACE.*{}".format(self.regex), logline) is not None:
            return True
        else:
            return False


class DuplicateProgramName(RegexMatcher):
    def __init__(self, name):
        super(DuplicateProgramName, self).__init__(
            "Duplicate program found for name {}".format(name))


class CycleDetected(RegexMatcher):
    def __init__(self, name):
        super(CycleDetected, self).__init__(
                "Found cycle involving process '{}'".format(name))

    def __str__(self):
        return self.regex


class ChildSpawned(RegexMatcher):
    def __init__(self, name):
        super(ChildSpawned, self).__init__(
                "Started child " + name)

    def __str__(self):
        return self.regex


class ChildExited(RegexMatcher):
    def __init__(self, name):
        super(ChildExited, self).__init__(
                "Child " + name + " exited successfully")

    def __str__(self):
        return self.regex


class ChildSleeping(RegexMatcher):
    def __init__(self, name):
        super(ChildSleeping, self).__init__(
                "Child " + name + " has finished and is going to sleep")

    def __str__(self):
        return self.regex


class ChildSkipped(RegexMatcher):
    def __init__(self, name):
        super(ChildSkipped, self).__init__(
                "Refusing to start child '" + name + "' which is currently "
                                                     "running")

    def __str__(self):
        return self.regex


class Exited(RegexMatcher):
    def __init__(self):
        super(Exited, self).__init__(
                "Exiting")

    def __str__(self):
        return self.regex


class ConfigError(RegexMatcher):
    def __init__(self):
        super(ConfigError, self).__init__(
                "Error in configuration file")

    def __str__(self):
        return self.regex


class ProgramConfigError(RegexMatcher):
    def __init__(self, name):
        super(ProgramConfigError, self).__init__(
            "Program " + name + " contains error.*")

    def __str__(self):
        return self.regex


class ChildCrashed(RegexMatcher):
    def __init__(self, name, rc):
        super(ChildCrashed, self).__init__(
                "Child " + name + " crashed with {}".format(rc))

    def __str__(self):
        return self.regex


class NoChildProcess:
    def __init__(self, name, test):
        try:
            open(PROJECT_ROOT + "/system-tests/child-dump/" + name + ".yml")
            test.fail("Child process '{}' did execute".format(name))
        except OSError:
            pass


class ChildProcess:

    def __init__(self, name, test, dump=False):
        self.test = test
        child_dumps = PROJECT_ROOT + "/system-tests/child-dump/"
        with open(child_dumps + name + ".yml") as stream:
            tree = yaml.load(stream)
            if dump:
                print(tree)
            program = tree['programs'][0]
            self.args = program['args']
            self.uid = program['uid']
            self.gid = program['gid']
            self.pty = program['pty']
            self.capabilities = set(program['capabilities'])
            self.env = program['env']
            self.workdir = program['workdir']

    def assert_arg(self, arg):
        self.test.assertTrue(arg in self.args,
                             arg + " not found in " + str(self.args))
        return self

    def assert_workdir(self, arg):
        self.test.assertEqual(arg, self.workdir)
        return self

    def assert_uid(self, uid):
        self.test.assertEqual(uid, self.uid, "uid mismatch")
        return self

    def assert_gid(self, gid):
        self.test.assertEqual(gid, self.gid, "gid mismatch")
        return self

    def assert_pty(self, pty):
        self.test.assertEqual(pty, self.pty, "pty mismatch")
        return self

    def assert_capabilities(self, caps):
        self.test.assertEqual(set(caps), self.capabilities)
        return self

    def assert_default_env(self):
        self.assert_env_is_keys(ChildProcess.get_default_env())
        return self

    def assert_env_is_keys(self, keys):
        self.test.assertEqual(keys, set(self.env.keys()))

    def assert_env_is(self, env):
        self.test.assertEqual(env, self.env)
        return self

    def assert_env(self, key, value):
        self.test.assertTrue(key in self.env)
        self.test.assertEqual(value, self.env[key])
        return self

    def assert_not_env(self, key):
        self.test.assertFalse(key in self.env)
        return self

    @staticmethod
    def get_default_env():
        return {
            "HOME",
            "LANG",
            "LANGUAGE",
            "LOGNAME",
            "PATH",
            "PWD",
            "SHELL",
            "TERM",
            "USER"
        }
