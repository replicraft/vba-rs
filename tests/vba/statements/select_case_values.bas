' INV: stmt.select-case
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/select-case-statement
' EXPECT: one
' EXPECT: two-three
' EXPECT: two-three
' EXPECT: four-five
' EXPECT: four-five
' EXPECT: else
Public Sub Main()
    Dim i As Long
    For i = 1 To 6
        Select Case i
            Case 1
                Debug.Print "one"
            Case 2, 3
                Debug.Print "two-three"
            Case 4 To 5
                Debug.Print "four-five"
            Case Else
                Debug.Print "else"
        End Select
    Next
End Sub
