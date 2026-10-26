' INV: vbal.5.3.1, stmt.on-error
' SOURCE: spec ms-vbal 5.3.1 invocation sets callee error policy to default
' EXPECT: callee-a
' EXPECT: caller-after
Sub Callee()
    Debug.Print "callee-a"
    Err.Raise 5
    Debug.Print "callee-b"
End Sub
Public Sub Main()
    On Error Resume Next
    Callee
    Debug.Print "caller-after"
End Sub
