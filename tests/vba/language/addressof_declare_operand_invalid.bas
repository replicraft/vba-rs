' INV: expr.addressof, vbal.5.6.16.8
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/addressof-operator
' ERROR: load
Declare PtrSafe Function GetTickCount Lib "kernel32" () As Long
Public Sub Main()
    Dim p As LongPtr
    p = AddressOf GetTickCount
End Sub
