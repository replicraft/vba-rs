' INV: stmt.option-private
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/option-private-statement
' EXPECT: ok
Option Private Module
Public Sub Main()
    Debug.Print "ok"
End Sub
