VERSION 5.00
Begin {C62A69F0-16DC-11CE-9E98-00AA00574A4F} CustomerForm
   Caption         =   "Customers"
   ClientHeight    =   3015
   ClientLeft      =   120
   ClientTop       =   465
   ClientWidth     =   4710
   OleObjectBlob   =   "CustomerForm.frx":0000
   StartUpPosition =   1  'CenterOwner
   Begin MSForms.CommandButton cmdOK
      Caption         =   "OK"
      Default         =   -1  'True
      Height          =   375
      Left            =   2640
      TabIndex        =   1
      Top             =   2520
      Width           =   855
   End
   Begin MSForms.TextBox txtName
      Height          =   285
      Left            =   120
      TabIndex        =   0
      Top             =   120
      Width           =   2175
   End
End
Attribute VB_Name = "CustomerForm"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit

Private mCancelled As Boolean

Public Property Get Cancelled() As Boolean
    Cancelled = mCancelled
End Property

Private Sub UserForm_Initialize()
    Me.txtName.Text = vbNullString
    Me.Caption = "Customers (" & Application.Name & ")"
End Sub

Private Sub cmdOK_Click()
    If Len(Trim$(Me.txtName.Text)) = 0 Then
        MsgBox "Please enter a name.", vbExclamation
        Me.txtName.SetFocus
        Exit Sub
    End If
    mCancelled = False
    Me.Hide
End Sub

Private Sub UserForm_QueryClose(Cancel As Integer, CloseMode As Integer)
    If CloseMode = vbFormControlMenu Then
        Cancel = True
        mCancelled = True
        Me.Hide
    End If
End Sub

Private Sub txtName_KeyPress(ByVal KeyAscii As MSForms.ReturnInteger)
    Select Case KeyAscii
        Case 13: cmdOK_Click
        Case 27: Unload Me
    End Select
End Sub
