' INV: vbal.6.1.2.8.1.11, func.iif
' SOURCE: spec ms-vbal 6.1.2.8.1.11 IIf always evaluates TruePart first and then FalsePart
' EXPECT: T
' EXPECT: F
' EXPECT: r=1
Private Function T() As Long
    Debug.Print "T"
    T = 1
End Function
Private Function F() As Long
    Debug.Print "F"
    F = 2
End Function
Public Sub Main()
    Debug.Print "r=" & IIf(True, T(), F())
End Sub
