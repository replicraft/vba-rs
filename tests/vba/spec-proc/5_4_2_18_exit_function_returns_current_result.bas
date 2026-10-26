' INV: vbal.5.4.2.18, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.18 Exit Function ends the function; the result variable's value is returned
' EXPECT: 7
' EXPECT: 0
Function F(ByVal stopEarly As Boolean) As Long
    If stopEarly Then
        F = 7
        Exit Function
    End If
    F = 100
End Function
Function G() As Long
    Exit Function
    G = 5
End Function
Public Sub Main()
    Debug.Print CStr(F(True))
    Debug.Print CStr(G())
End Sub
