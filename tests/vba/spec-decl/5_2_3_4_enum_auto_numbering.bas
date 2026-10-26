' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum member values: first is 0, others previous plus one
' EXPECT: 0 1 2
' EXPECT: 5 6 20 21
' EXPECT: -3 -2 -1 0
Private Enum Plain
    A
    B
    C
End Enum
Private Enum Mixed
    D = 5
    E
    F = 20
    G
End Enum
Private Enum Neg
    H = -3
    I
    J
    K
End Enum
Public Sub Main()
    Debug.Print CStr(A) & " " & CStr(B) & " " & CStr(C)
    Debug.Print CStr(D) & " " & CStr(E) & " " & CStr(F) & " " & CStr(G)
    Debug.Print CStr(H) & " " & CStr(I) & " " & CStr(J) & " " & CStr(K)
End Sub
