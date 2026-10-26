' INV: vbal.5.4.2.14, vbal.5.4.2.15
' SOURCE: spec ms-vbal 5.4.2.14 each invocation has its own GoSub resumption list
' EXPECT: callee-err=3
' EXPECT: return-ok
Sub Callee()
    Return
End Sub
Public Sub Main()
    GoSub L
    Debug.Print "return-ok"
    Exit Sub
L:
    On Error Resume Next
    Callee
    Debug.Print "callee-err=" & CStr(Err.Number)
    On Error GoTo 0
    Return
End Sub
