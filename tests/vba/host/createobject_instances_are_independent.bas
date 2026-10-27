' INV: func.createobject, vbal.6.1.2.8.1.4
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createobject-function
' METHOD: each call creates a new instance (the page: CreateObject creates an instance); counts by hand
' GRANT: com-create
' EXPECT:  2 
' EXPECT:  0 
Public Sub Main()
    Dim a As Object, b As Object
    Set a = CreateObject("Scripting.Dictionary")
    Set b = CreateObject("Scripting.Dictionary")
    a.Add 1, "x"
    a.Add 2, "y"
    Debug.Print a.Count
    Debug.Print b.Count
End Sub
