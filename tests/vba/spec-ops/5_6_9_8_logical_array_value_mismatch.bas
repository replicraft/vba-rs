' INV: vbal.5.6.9.8, op.or
' SOURCE: spec ms-vbal 5.6.9.8 logical operator with array value type raises 13
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim v As Variant, r As Variant
    v = Array(1)
    Debug.Print "before"
    r = (1 Or v)
    Debug.Print "after"
End Sub
