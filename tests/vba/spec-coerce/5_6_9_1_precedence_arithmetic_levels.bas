' INV: vbal.5.6.9.1
' SOURCE: spec ms-vbal 5.6.9.1 Operator precedence and associativity, exponentiation > unary - > multiplicative > integer division > Mod > additive
' EXPECT: -2^2=-4
' EXPECT: 2*3^2=18
' EXPECT: 7\2*2=1
' EXPECT: 7 Mod 4\3=0
' EXPECT: 1+7 Mod 4=4
' EXPECT: 1+2*3=7
' EXPECT: -3*2=-6
' EXPECT: 10-2*3=4
Public Sub Main()
    Debug.Print "-2^2=" & CStr(-2 ^ 2)
    Debug.Print "2*3^2=" & CStr(2 * 3 ^ 2)
    Debug.Print "7\2*2=" & CStr(7 \ 2 * 2)
    Debug.Print "7 Mod 4\3=" & CStr(7 Mod 4 \ 3)
    Debug.Print "1+7 Mod 4=" & CStr(1 + 7 Mod 4)
    Debug.Print "1+2*3=" & CStr(1 + 2 * 3)
    Debug.Print "-3*2=" & CStr(-3 * 2)
    Debug.Print "10-2*3=" & CStr(10 - 2 * 3)
End Sub
