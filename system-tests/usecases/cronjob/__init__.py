from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace().that(
            Sequential(
                ChildSpawned("waiter"),
                Parallel(
                    Sequential(
                        ChildSpawned("echo"),
                        ChildSleeping("echo"),
                        ChildSpawned("echo"),
                        ChildSleeping("echo"),
                    ),
                    Sequential(
                        ChildSpawned("non-reentrant"),
                        ChildSkipped("non-reentrant"),
                        ChildSleeping("non-reentrant"),
                    )
                ),
                ChildExited("waiter"),
                Exited()
            )
        )

