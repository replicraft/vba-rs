' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational array value type raises 13
' EXPECT: before
' ERROR: 13
Public Sub Main()
    Dim v As Variant, r As Variant
    v = Array(1, 2)
    Debug.Print "before"
    r = (v = 1)
    Debug.Print "after"
End Sub
