' INV: func.dateserial
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateserial-function
' ERROR: 5
Public Sub Main()
    Debug.Print DateSerial(10000, 1, 1)
End Sub
