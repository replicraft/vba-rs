' INV: stmt.option-explicit
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/option-explicit-statement
' EXPECT:  1 
Option Explicit
Public Sub Main()
    Dim x As Long
    x = 1
    Debug.Print x
End Sub
