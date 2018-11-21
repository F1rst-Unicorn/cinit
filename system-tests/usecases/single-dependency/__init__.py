from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace().that(
            Sequential(
                ChildSpawned("ping"),
                ChildExited("ping"),
                ChildSpawned("failping"),
                ChildExited("failping"),
                Exited()
            )
        )

        ChildProcess("ping", self)\
            .assert_arg("-c 4")\
            .assert_arg("google.ch")\
            .assert_uid(0)\
            .assert_gid(0)\
            .assert_default_env()\
            .assert_pty(False)\
            .assert_capabilities({"CAP_NET_RAW"})

        ChildProcess("failping", self)\
            .assert_arg("-c 4")\
            .assert_arg("google.ch")\
            .assert_uid(1000)\
            .assert_gid(100)\
            .assert_default_env()\
            .assert_pty(False)\
            .assert_capabilities({})


