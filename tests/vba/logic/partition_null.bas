' INV: func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Partition(Null, 0, 10, 5))
End Sub
