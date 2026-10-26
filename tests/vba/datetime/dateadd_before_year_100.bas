' INV: func.dateadd
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateadd-function
' ERROR: 5
Public Sub Main()
    Debug.Print DateAdd("yyyy", -2000, DateSerial(1000, 1, 1))
End Sub
