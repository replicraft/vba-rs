' INV: stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 Select Case statement expression evaluated once
' EXPECT:  1 
Dim g As Long
Function F() As Long
    g = g + 1
    F = 2
End Function
Public Sub Main()
    Select Case F()
        Case 1
        Case 2
        Case 3
    End Select
    Debug.Print g
End Sub
