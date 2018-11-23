from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace().that(
            Sequential(
                ChildSpawned("first"),
                ChildCrashed("first", 42),
                Exited()
            )
        )

        ChildProcess("first", self)\
            .assert_uid(0)\
            .assert_gid(0)\
            .assert_default_env()\
            .assert_pty(False)

        NoChildProcess("second", self)
