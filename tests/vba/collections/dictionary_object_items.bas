' INV: obj.dictionary, type.dictionary
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dictionary-object
' EXPECT: 42
' EXPECT: 42
' EXPECT: True
' GRANT: com-create
'#MODULE Box class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim d As Object, b As New Box, o As Object
    b.V = 42
    Set d = CreateObject("Scripting.Dictionary")
    ' Items can be any form of data, including objects.
    d.Add "box", b
    Debug.Print d("box").V & ""
    Set o = d.Item("box")
    Debug.Print o.V & ""
    d.Remove "box"
    ' The caller's variable still holds the object.
    Debug.Print Not b Is Nothing
End Sub
