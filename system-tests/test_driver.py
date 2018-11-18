import re
import os
import sys
import unittest
import subprocess

UUT_PATH = os.environ['UUT']
PROJECT_ROOT = os.environ['PROJECT_ROOT']

class CinitTest(unittest.TestCase):

    def tearDown(self):
        child_dumps = PROJECT_ROOT + "/system-tests/child-dump/"
        for file in os.listdir(child_dumps):
            os.unlink(child_dumps + file)

    def run_cinit(self, test_dir):
        cinit = subprocess.Popen([
                        UUT_PATH,
                        "--verbose",
                        "--config",
                        test_dir + "/config"],
                stdout=subprocess.PIPE,
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                cwd=PROJECT_ROOT)

        cinit.wait()
        output = cinit.stdout.read().decode("utf-8").split("\n");
        cinit.stdout.close()

        self.trace = Trace(self, output)
        self.children = {}

        child_dumps = PROJECT_ROOT + "/system-tests/child-dump/"

    def assert_on_trace(self):
        return self.trace

    def get_test_dir(self, path):
        return os.path.dirname(os.path.abspath(path))

class Trace:

    def __init__(self, test, trace):
        self.trace = trace
        self.test = test
        self.index = 0

    def that(self, assertion):
        while self.index < len(self.trace):
            if assertion.matches(self.trace[self.index]):
                return self
            self.index = self.index + 1

        for line in self.trace:
            print(line)

        self.test.fail("Event '" + str(assertion) + "' has not occured")

    def then(self, assertion):
        return self.that(assertion)

class Assert:

    def matches():
        return False

class RegexMatcher(Assert):

    def __init__(self, regex, test):
        self.regex = regex
        self.test = test

    def __str__(self):
        return self.regex

    def matches(self, logline):
        if None != re.fullmatch(
                ".*TRACE.*{}".format(self.regex), logline):
            return True
        else:
            return False

class CycleDetected(RegexMatcher):
    def __init__(self, test):
        super(CycleDetected, self).__init__(
                "No runnable processes found, check for cycles", test)

class ChildSpawned(RegexMatcher):
    def __init__(self, name, test):
        super(ChildSpawned, self).__init__(
                "Started child " + name, test)

class ChildExited(RegexMatcher):
    def __init__(self, name, test):
        super(ChildExited, self).__init__(
                "Child " + name + " exited successfully", test)

class ChildCrashed(RegexMatcher):
    def __init__(self, name, test):
        super(ChildCrashed, self).__init__(
                "Child " + name + " crashed with \d+", test)

class ChildProcess:

    def __init__(name):
        pass

    def assert_env(self, key, value):
        pass

    def assert_uid(uid):
        pass

    def assert_gid(gid):
        pass


