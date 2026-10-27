' INV: func.getobject, vbal.6.1.2.8.1.9
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/getobject-function
' METHOD: the page: if no object of the specified class exists, an error occurs; the number is not stated, so only that a trappable error occurs is checked
' GRANT: com-create
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim o As Object
    On Error Resume Next
    Set o = GetObject(, "No.Such.Application")
    Debug.Print Err.Number <> 0
    Debug.Print o Is Nothing
End Sub
