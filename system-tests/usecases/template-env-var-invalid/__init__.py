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
            .assert_env("FOO", "bar")\
            .assert_env("USER", "root")\
            .assert_env("BAR", "{{ USER -{{ FOO }}")

