' INV: stmt.select-case
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/select-case-statement
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
