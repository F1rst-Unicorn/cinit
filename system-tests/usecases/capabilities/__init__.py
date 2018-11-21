from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace()                                              \
                .that(Sequential(
                        Parallel(
                            Sequential(
                                ChildSpawned("some-caps"),
                                ChildExited("some-caps")
                            ),
                            Sequential(
                                ChildSpawned("no-caps"),
                                ChildExited("no-caps")
                            )
                        ),
                        Exited()))                              \

        ChildProcess("some-caps", self) \
            .assert_arg("-o") \
            .assert_arg("system-tests/child-dump/some-caps.yml") \
            .assert_uid(1000) \
            .assert_gid(100) \
            .assert_default_env() \
            .assert_pty(True) \
            .assert_capabilities({"CAP_NET_RAW",
                                  "CAP_KILL"})

        ChildProcess("no-caps", self) \
            .assert_arg("-o") \
            .assert_arg("system-tests/child-dump/no-caps.yml") \
            .assert_uid(1000) \
            .assert_gid(100) \
            .assert_default_env() \
            .assert_pty(True) \
            .assert_capabilities({})
