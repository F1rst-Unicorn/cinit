from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace()                                              \
                .that(Sequential(
                        ChildSpawned("first"),
                        ChildExited("first"),
                        Exited()))                              \


        ChildProcess("first", self)\
            .assert_arg("-o")\
            .assert_arg("system-tests/child-dump/first.yml")\
            .assert_uid(1000)\
            .assert_gid(100)\
            .assert_pty(True)\
            .assert_default_env()\
            .assert_caps({"CAP_KILL", "CAP_NET_RAW"})
