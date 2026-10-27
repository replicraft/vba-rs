' INV: func.createobject, vbal.6.1.2.8.1.4
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createobject-function
' METHOD: Class is not marked optional in the syntax, so omitting it does not compile
' GRANT: com-create
' ERROR: load
Public Sub Main()
    Dim d As Object
    Set d = CreateObject()
End Sub
