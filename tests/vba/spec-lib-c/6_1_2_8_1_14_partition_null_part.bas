' INV: vbal.6.1.2.8.1.14, func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition returns Null if any part is Null
' EXPECT: True True True True
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print IsNull(Partition(n, 0, 99, 5)) & " " & IsNull(Partition(5, n, 99, 5)) & " " & IsNull(Partition(5, 0, n, 5)) & " " & IsNull(Partition(5, 0, 99, n))
End Sub
