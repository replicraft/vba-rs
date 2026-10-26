' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like star validates rest of pattern even when later match would fail
' EXPECT: before
' ERROR: 93
Public Sub Main()
    Dim r As Boolean
    Debug.Print "before"
    r = "q" Like "*[b"
    Debug.Print "after"
End Sub
