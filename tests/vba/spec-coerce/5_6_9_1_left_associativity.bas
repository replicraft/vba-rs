' INV: vbal.5.6.9.1
' SOURCE: spec ms-vbal 5.6.9.1 Operator precedence and associativity, all binary operators are left-associative (including ^)
' EXPECT: 2-3-4=-5
' EXPECT: 100/10/5=2
' EXPECT: 2^3^2=64
' EXPECT: 2^3^2 parens=512
' EXPECT: 100\10\5=2
' EXPECT: 17 Mod 10 Mod 4=3
' EXPECT: 8/2*2=8
' EXPECT: concat=abc
Public Sub Main()
    Debug.Print "2-3-4=" & CStr(2 - 3 - 4)
    Debug.Print "100/10/5=" & CStr(100 / 10 / 5)
    Debug.Print "2^3^2=" & CStr(2 ^ 3 ^ 2)
    Debug.Print "2^3^2 parens=" & CStr(2 ^ (3 ^ 2))
    Debug.Print "100\10\5=" & CStr(100 \ 10 \ 5)
    Debug.Print "17 Mod 10 Mod 4=" & CStr(17 Mod 10 Mod 4)
    Debug.Print "8/2*2=" & CStr(8 / 2 * 2)
    Debug.Print "concat=" & "a" & "b" & "c"
End Sub
