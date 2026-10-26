' INV: stmt.let
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/let-statement
' EXPECT:  4 
Public Sub Main()
    Dim x As Long
    Let x = 4
    Debug.Print x
End Sub
