' INV: vbal.5.6.2.1
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, the function's own name read inside the function refers to the return value rather than recursing
' XFAIL: wrong pending
' EXPECT: f=6
' EXPECT: calls=1
Dim calls As Long
Function F() As Long
    calls = calls + 1
    F = 5
    F = F + 1
End Function
Public Sub Main()
    Debug.Print "f=" & CStr(F)
    Debug.Print "calls=" & CStr(calls)
End Sub
