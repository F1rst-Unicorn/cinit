from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace().that(
            Sequential(
                ChildSpawned("program"),
                ChildExited("program"),
                Exited()
            )
        )

        ChildProcess("program", self)\
            .assert_arg("hello world plus appendix")\
            .assert_arg("{{ ENV_VAR } plus appendix")


