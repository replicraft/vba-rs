' INV: obj.dictionary, type.dictionary
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/remove-method-dictionary-object
' EXPECT: a,b,c
' EXPECT: a,c
' GRANT: com-create
Public Sub Main()
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "a", "Athens"
    d.Add "b", "Belgrade"
    d.Add "c", "Cairo"
    Debug.Print JoinKeys(d)
    d.Remove "b"
    Debug.Print JoinKeys(d)
End Sub

Private Function JoinKeys(ByVal d As Object) As String
    Dim k As Variant, s As String
    For Each k In d.Keys
        s = s & IIf(s = "", "", ",") & k
    Next k
    JoinKeys = s
End Function
