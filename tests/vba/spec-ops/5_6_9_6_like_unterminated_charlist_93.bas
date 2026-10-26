' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like incomplete charlist raises 93 when reached
' EXPECT: before
' ERROR: 93
Public Sub Main()
    Dim r As Boolean
    Debug.Print "before"
    r = "a" Like "a[bc"
    Debug.Print "after"
End Sub
