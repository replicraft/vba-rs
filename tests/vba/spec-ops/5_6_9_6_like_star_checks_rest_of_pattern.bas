' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like star immediately validates the rest of the pattern (93)
' EXPECT: before
' ERROR: 93
Public Sub Main()
    Dim r As Boolean
    Debug.Print "before"
    r = "abc" Like "a*[b"
    Debug.Print "after"
End Sub
