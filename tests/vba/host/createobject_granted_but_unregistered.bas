' INV: func.createobject, vbal.6.1.2.8.1.4
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createobject-function
' ERROR: 429
' GRANT: com-create
Public Sub Main()
    Dim o As Object
    Set o = CreateObject("No.Such.Class")
End Sub
