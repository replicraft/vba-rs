' INV: stmt.declare
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/declare-statement
' ERROR: 70
Declare PtrSafe Function GetTickCount Lib "kernel32" () As Long
Public Sub Main()
    Debug.Print GetTickCount()
End Sub
