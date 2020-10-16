#  cinit: process initialisation program for containers
#  Copyright (C) 2019 The cinit developers
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#
#  You should have received a copy of the GNU General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.

from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace().that(
            Sequential(
                Parallel(
                    ChildSpawned("waiter"),
                    ChildSpawned("dependency"),
                ),
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
                    ),
                    Sequential(
                        Parallel(
                            Sequential(
                                ChildSpawned("independent-cronjob"),
                                ChildSleeping("independent-cronjob"),
                            ),
                            ChildCronjobSkipped("dependent-cronjob"),
                        ),
                        Parallel(
                            Sequential(
                                ChildSpawned("independent-cronjob"),
                                ChildSleeping("independent-cronjob"),
                            ),
                            Sequential(
                                ChildExited("dependency"),
                                ChildSpawned("dependent-cronjob"),
                                ChildSleeping("dependent-cronjob"),
                            ),
                        ),
                    ),
                ),
                ChildExited("waiter"),
                Exited()
            )
        )

