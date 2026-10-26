' INV: vbal.5.6.9.7, op.is
' SOURCE: spec ms-vbal 5.6.9.7 Is operator with non-object value type raises 424
' EXPECT: before
' ERROR: 424
Public Sub Main()
    Dim v As Variant, o As Object, r As Boolean
    v = 5
    Debug.Print "before"
    r = (v Is o)
    Debug.Print "after"
End Sub
