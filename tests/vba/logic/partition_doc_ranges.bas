' INV: func.partition
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/partition-function
' EXPECT: :-1
' EXPECT: 0:4
' EXPECT: 95:99
' EXPECT: 100:
' EXPECT: :19
' EXPECT: 190:199
' EXPECT: 1000:1010
' EXPECT: 1011:
' XFAIL: unsupported Partition is not defined ('Sub or Function not defined')
Public Sub Main()
    Debug.Print Replace(Partition(-1, 0, 99, 5), " ", "")
    Debug.Print Replace(Partition(0, 0, 99, 5), " ", "")
    Debug.Print Replace(Partition(99, 0, 99, 5), " ", "")
    Debug.Print Replace(Partition(100, 0, 99, 5), " ", "")
    Debug.Print Replace(Partition(19, 20, 199, 10), " ", "")
    Debug.Print Replace(Partition(190, 20, 199, 10), " ", "")
    Debug.Print Replace(Partition(1010, 100, 1010, 20), " ", "")
    Debug.Print Replace(Partition(1011, 100, 1010, 20), " ", "")
End Sub
