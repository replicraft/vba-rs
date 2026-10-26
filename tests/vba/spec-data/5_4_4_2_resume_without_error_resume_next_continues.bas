' INV: vbal.5.4.4.2, stmt.resume, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.2 Resume with no active error is error 20, which On Error Resume Next skips
' EXPECT:  20 
' EXPECT: after
Public Sub Main()
    On Error Resume Next
    Resume
    Debug.Print Err.Number
    Debug.Print "after"
End Sub
