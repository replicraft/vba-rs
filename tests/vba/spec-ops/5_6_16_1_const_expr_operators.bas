' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expressions with literals, operators, and references to constants
' EXPECT: 14
' EXPECT: 5
' EXPECT: 3
' EXPECT: 1
' EXPECT: ab12
' EXPECT: True
' EXPECT: -9
' EXPECT: 8
' EXPECT: 6
' EXPECT: 14
Private Const A As Long = 2 + 3 * 4
Private Const B As Long = (A - 4) \ 2
Private Const C As Long = A Mod 11
Private Const D As Long = 7 \ 4
Private Const S As String = "ab" & 12
Private Const T As Boolean = 3 > 2 And Not (1 = 2)
Private Const U As Long = -(3 ^ 2)
Private Const V As Long = 12 And 10
Private Const W As Long = 12 Xor 10
Private Const X As Long = 12 Or 10
Public Sub Main()
    Debug.Print CStr(A)
    Debug.Print CStr(B)
    Debug.Print CStr(C)
    Debug.Print CStr(D)
    Debug.Print S
    Debug.Print CStr(T)
    Debug.Print CStr(U)
    Debug.Print CStr(V)
    Debug.Print CStr(W)
    Debug.Print CStr(X)
End Sub
