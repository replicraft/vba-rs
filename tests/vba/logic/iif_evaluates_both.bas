' INV: func.iif
' SOURCE: spec ms-vbal 6.1.2.8.1.11 IIf
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
