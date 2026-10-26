' INV: func.partition
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/partition-function
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Partition(Null, 0, 10, 5))
End Sub
