' INV: vbal.5.6.9.3.6, op.mod
' SOURCE: spec ms-vbal 5.6.9.3.6 Mod Operator, remainder of the truncated quotient; exact quotient gives 0
' EXPECT: 7 Mod 4=3
' EXPECT: 8 Mod 4=0
' EXPECT: 4 Mod 7=4
' EXPECT: 7 Mod -4=3
' EXPECT: 0 Mod 5=0
' EXPECT: 10 Mod 3=1
Public Sub Main()
    Dim a As Long, b As Long
    a = 7: b = 4
    Debug.Print "7 Mod 4=" & CStr(a Mod b)
    a = 8: b = 4
    Debug.Print "8 Mod 4=" & CStr(a Mod b)
    a = 4: b = 7
    Debug.Print "4 Mod 7=" & CStr(a Mod b)
    a = 7: b = -4
    Debug.Print "7 Mod -4=" & CStr(a Mod b)
    a = 0: b = 5
    Debug.Print "0 Mod 5=" & CStr(a Mod b)
    a = 10: b = 3
    Debug.Print "10 Mod 3=" & CStr(a Mod b)
End Sub
