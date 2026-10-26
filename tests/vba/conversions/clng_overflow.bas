' INV: func.clng
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' ERROR: 6
Public Sub Main()
    Debug.Print CLng(2147483648#)
End Sub
