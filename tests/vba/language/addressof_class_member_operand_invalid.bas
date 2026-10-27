' INV: expr.addressof, vbal.5.6.16.8
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/addressof-operator
' ERROR: load
'#MODULE Worker class
Public Sub Run()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim p As LongPtr
    Dim w As New Worker
    p = AddressOf w.Run
End Sub
