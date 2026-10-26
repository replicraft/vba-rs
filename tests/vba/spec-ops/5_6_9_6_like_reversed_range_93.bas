' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like range whose end is less than its start raises 93
' EXPECT: before
' ERROR: 93
Public Sub Main()
    Dim r As Boolean
    Debug.Print "before"
    r = "b" Like "[z-a]"
    Debug.Print "after"
End Sub
