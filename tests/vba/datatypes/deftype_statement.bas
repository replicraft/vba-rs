' INV: stmt.deftype
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/deftype-statements
' EXPECT: IntegerStringEmpty
DefInt A-C
DefStr S
Public Sub Main()
    Dim apple, sorted, zed
    Debug.Print TypeName(apple); TypeName(sorted); TypeName(zed)
End Sub
