' INV: vbal.5.6.9.4, op.concat
' SOURCE: spec ms-vbal 5.6.9.4 & operator, non-Byte array value type raises 13
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim s As String
    Dim v As Variant
    v = Array(1, 2)
    Debug.Print "before"
    s = "a" & v
    Debug.Print "after"
End Sub
