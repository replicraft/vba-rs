' INV: vbal.5.4.2.22, stmt.end
' SOURCE: spec ms-vbal 5.4.2.22 End terminates execution immediately, also from a nested call
' EXPECT: before
' EXPECT: in-callee
Sub Callee()
    Debug.Print "in-callee"
    End
    Debug.Print "never-callee"
End Sub
Public Sub Main()
    Debug.Print "before"
    Callee
    Debug.Print "never-main"
End Sub
