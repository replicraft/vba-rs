' INV: func.iif
' SOURCE: derived IIf evaluates both branches (documented for Choose/Switch; IIf is a function call)
' EXPECT:  1  2 
Dim g As Long
Function Bump() As Long
    g = g + 1
    Bump = g
End Function
Public Sub Main()
    Dim n As Long
    n = IIf(True, Bump(), Bump())
    Debug.Print n; g
End Sub
