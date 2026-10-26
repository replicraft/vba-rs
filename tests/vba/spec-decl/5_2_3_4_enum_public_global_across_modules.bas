' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 public and Global enums: type and members accessible in the project
' EXPECT: 1 2 11 12
Public Sub Main()
    Dim x As Pub
    x = P2
    Debug.Print CStr(Other.P1) & " " & CStr(x) & " " & CStr(G1) & " " & CStr(G2)
End Sub
'#MODULE Other standard
Public Enum Pub
    P1 = 1
    P2
End Enum
Global Enum Glob
    G1 = 11
    G2
End Enum
