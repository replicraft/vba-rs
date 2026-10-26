' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call invokes a function and discards the result; Call with argument list
' EXPECT: f(3)
' EXPECT: s 1 2
Function F(ByVal n As Long) As Long
    Debug.Print "f(" & CStr(n) & ")"
    F = n
End Function
Sub S(a As Long, b As Long)
    Debug.Print "s " & CStr(a) & " " & CStr(b)
End Sub
Public Sub Main()
    Call F(3)
    Call S(1, 2)
End Sub
