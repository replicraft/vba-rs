' INV: func.getobject
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/getobject-function
' ERROR: 70
Public Sub Main()
    Dim o As Object
    Set o = GetObject("/some/file.xls")
End Sub
