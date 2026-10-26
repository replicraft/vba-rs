' INV: stmt.for-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/fornext-statement
' EXPECT:  0 
' EXPECT:  0.25 
' EXPECT:  0.5 
' EXPECT:  0.75 
' EXPECT:  1 
Public Sub Main()
    Dim x As Double
    For x = 0 To 1 Step 0.25
        Debug.Print x
    Next
End Sub
