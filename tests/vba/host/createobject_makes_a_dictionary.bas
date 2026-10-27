' INV: func.createobject, vbal.6.1.2.8.1.4
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createobject-function
' METHOD: by hand; the ProgID is matched without regard to case
' GRANT: com-create
' EXPECT: Dictionary
' EXPECT:  1 
' EXPECT: Dictionary
' EXPECT: False
Public Sub Main()
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "k", 1
    Debug.Print TypeName(d)
    Debug.Print d.Count
    Dim e As Object
    Set e = CreateObject("scripting.DICTIONARY")
    Debug.Print TypeName(e)
    Debug.Print d Is e
End Sub
