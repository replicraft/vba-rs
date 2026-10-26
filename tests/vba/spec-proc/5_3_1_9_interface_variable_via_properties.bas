' INV: vbal.5.3.1.9, stmt.implements
' SOURCE: spec ms-vbal 5.3.1.9 interface variable implemented by property declarations; dispatch to implementer
' EXPECT: title=hello
'#MODULE IThing class
Public Title As String
'#MODULE Impl class
Implements IThing
Private store As String
Private Property Get IThing_Title() As String
    IThing_Title = store
End Property
Private Property Let IThing_Title(ByVal v As String)
    store = v
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim t As IThing
    Set t = New Impl
    t.Title = "hello"
    Debug.Print "title=" & t.Title
End Sub
