' INV: vbal.5.5.1.2.1.1
' SOURCE: derived ms-vbal 5.5.1.2.1.1 a value exactly halfway between two integers rounds to the even one (banker's rounding)
' EXPECT: 0 2 2 4 4
' EXPECT: 0 -2 -2
' EXPECT: 2 4
Public Sub Main()
    Debug.Print CStr(CInt(0.5)) & " " & CStr(CInt(1.5)) & " " & CStr(CInt(2.5)) & " " & CStr(CLng(3.5)) & " " & CStr(CLng(4.5))
    Debug.Print CStr(CInt(-0.5)) & " " & CStr(CInt(-1.5)) & " " & CStr(CInt(-2.5))
    Dim b As Byte, l As Long
    b = 2.5
    l = 3.5
    Debug.Print CStr(b) & " " & CStr(l)
End Sub
