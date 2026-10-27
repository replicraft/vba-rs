' INV: func.getobject, vbal.6.1.2.8.1.9
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/getobject-function
' METHOD: the page states that class is required when pathname is omitted; the error number is not stated, so only that a trappable error occurs is checked
' GRANT: com-create
' EXPECT: True
Public Sub Main()
    Dim o As Object
    On Error Resume Next
    Set o = GetObject()
    Debug.Print Err.Number <> 0
End Sub
