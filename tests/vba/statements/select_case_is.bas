' INV: stmt.select-case
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/select-case-statement
' EXPECT: small
' EXPECT: mid
' EXPECT: large
Public Sub Main()
    Dim i As Long
    For i = 5 To 25 Step 10
        Select Case i
            Case Is < 10
                Debug.Print "small"
            Case Is >= 20
                Debug.Print "large"
            Case Else
                Debug.Print "mid"
        End Select
    Next
End Sub
