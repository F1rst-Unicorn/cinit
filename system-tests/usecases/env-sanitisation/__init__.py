from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace().that(
            Sequential(
                ChildSpawned("first"),
                ChildExited("first"),
                Exited()
            )
        )

        ChildProcess("first", self)\
            .assert_uid(1409)\
            .assert_gid(1409)\
            .assert_env("USER", "testuser")\
            .assert_env("LOGNAME", "testuser")\
            .assert_env("SHELL", "/bin/sh")\
            .assert_env("PWD", "/home/testuser")\
            .assert_env("HOME", "/home/testuser")\
