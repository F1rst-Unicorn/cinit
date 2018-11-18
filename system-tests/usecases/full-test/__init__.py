from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace()                                              \
                .that(Sequential(
                        ChildSpawned("first"),
                        ChildExited("first")))                              \
                .then(Parallel(
                        Sequential(
                                ChildSpawned("second"),
                                ChildExited("second")),
                        Sequential(
                                ChildSpawned("third"),
                                ChildExited("third"))))                     \
                .then(Sequential(
                        ChildSpawned("fourth"),
                        ChildExited("fourth")))                             \


        ChildProcess("first", self)\
            .assert_uid(0)\
            .assert_gid(0)\
            .assert_pty(True)

        ChildProcess("second", self)\
            .assert_uid(1000)\
            .assert_gid(100)\
            .assert_pty(False)

        ChildProcess("third", self)\
            .assert_uid(1000)\
            .assert_gid(100)\
            .assert_pty(True)

        ChildProcess("fourth", self)\
            .assert_uid(1000)\
            .assert_gid(100)\
            .assert_pty(True)
