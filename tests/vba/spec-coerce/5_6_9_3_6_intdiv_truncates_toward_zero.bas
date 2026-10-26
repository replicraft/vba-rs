' INV: vbal.5.6.9.3.6, op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 \ Operator, the quotient is rounded toward zero
' EXPECT: 7\2=3
' EXPECT: -7\2=-3
' EXPECT: 7\-2=-3
' EXPECT: -7\-2=3
' EXPECT: 6\3=2
' EXPECT: 1\2=0
' EXPECT: -1\2=0
Public Sub Main()
    Dim a As Long, b As Long
    a = 7: b = 2
    Debug.Print "7\2=" & CStr(a \ b)
    a = -7: b = 2
    Debug.Print "-7\2=" & CStr(a \ b)
    a = 7: b = -2
    Debug.Print "7\-2=" & CStr(a \ b)
    a = -7: b = -2
    Debug.Print "-7\-2=" & CStr(a \ b)
    a = 6: b = 3
    Debug.Print "6\3=" & CStr(a \ b)
    a = 1: b = 2
    Debug.Print "1\2=" & CStr(a \ b)
    a = -1: b = 2
    Debug.Print "-1\2=" & CStr(a \ b)
End Sub
