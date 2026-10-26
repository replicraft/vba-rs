' INV: func.partition
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/partition-function
' EXPECT: 100:100
Public Sub Main()
    Debug.Print Replace(Partition(100, 0, 1000, 1), " ", "")
End Sub
