' INV: vbal.5.6.9.3.6, op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 \ Operator, floating-point operands are Let-coerced to an integral effective type (Banker's rounding) before dividing
' EXPECT: 7.5\2=4
' EXPECT: 6.5\2=3
' EXPECT: 7\2.5=3
' EXPECT: 7\3.5=1
' EXPECT: 5.5 Mod 2=0
' EXPECT: 6.5 Mod 4=2
' EXPECT: 7 Mod 2.5=1
' EXPECT: str=2
Public Sub Main()
    Dim d As Double
    Dim n As Long
    d = 7.5: n = 2
    Debug.Print "7.5\2=" & CStr(d \ n)
    d = 6.5
    Debug.Print "6.5\2=" & CStr(d \ n)
    d = 2.5: n = 7
    Debug.Print "7\2.5=" & CStr(n \ d)
    d = 3.5
    Debug.Print "7\3.5=" & CStr(n \ d)
    d = 5.5: n = 2
    Debug.Print "5.5 Mod 2=" & CStr(d Mod n)
    d = 6.5: n = 4
    Debug.Print "6.5 Mod 4=" & CStr(d Mod n)
    d = 2.5: n = 7
    Debug.Print "7 Mod 2.5=" & CStr(n Mod d)
    Debug.Print "str=" & CStr("7" \ "3")
End Sub
