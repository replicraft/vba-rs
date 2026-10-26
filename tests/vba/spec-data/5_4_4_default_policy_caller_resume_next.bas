' INV: vbal.5.4.4, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 caller's Resume Next policy continues after the call statement
' EXPECT: inner-before
' EXPECT: after-call
Private Sub Inner()
    Debug.Print "inner-before"
    Error 5
    Debug.Print "inner-after"
End Sub
Public Sub Main()
    On Error Resume Next
    Inner
    Debug.Print "after-call"
End Sub
