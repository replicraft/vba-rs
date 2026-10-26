' INV: func.qbcolor
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/qbcolor-function
' EXPECT:  128 
' XFAIL: unsupported QBColor is not defined ('Sub or Function not defined')
Public Sub Main()
    Debug.Print QBColor(4)
End Sub
